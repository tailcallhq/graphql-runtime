package tailcall.cli

import tailcall.cli.CommandADT.Remote
import tailcall.cli.service.CommandExecutor
import zio.cli._

object CommandDoc {

  val command: Command[CommandADT] = Command("tc", Options.none).subcommands(
    Command(
      "check",
      CustomOptions.remoteOption.optional ++
        Options.boolean("n-plus-one-queries").alias("npo") ++
        CustomOptions.blueprintOptions,
      Args.file.repeat1,
    ).withHelp("Validate a composition spec, display its status when remote is passed.").map {
      case (remote, nPlusOne, blueprintOptions) -> config => CommandADT
          .Check(config, remote, nPlusOne, blueprintOptions)
    },

    // publish
    Command("publish", CustomOptions.remoteDefaultOption, Args.file.repeat1)
      .withHelp("Publish the configuration file to the remote environment.").map { case (remote, config) =>
        Remote(remote, Remote.Publish(config))
      },

    // drop
    Command("drop", CustomOptions.remoteDefaultOption, CustomArgs.digestArgs)
      .withHelp("Remove the composition spec from the remote environments using its SHA-256 hash.").map {
        case (remote, digest) => Remote(remote, Remote.Drop(digest))
      },

    // list
    Command(
      "list",
      CustomOptions.remoteDefaultOption ++
        CustomOptions.integerOption("offset").withDefault(0) ++
        CustomOptions.integerOption("limit").withDefault(Int.MaxValue),
    ).withHelp("List all published composition specs on the remote address.").map { case (remote, offset, limit) =>
      Remote(remote, Remote.ListAll(offset = offset, limit = limit))
    },

    // info
    Command("show", CustomOptions.remoteDefaultOption ++ CustomOptions.blueprintOptions, CustomArgs.digestArgs)
      .withHelp("Display info for a composition spec using its SHA-256 hash on the remote server.")
      .map { case (remote, blueprintOptions) -> digest =>
        Remote(remote, Remote.Show(digest = digest, options = blueprintOptions))
      },

    // generate
    Command(
      "generate",
      CustomOptions.sourceFormat ++ CustomOptions.targetFormat ++ Options.file("write").optional,
      Args.file.repeat1,
    ).withHelp("Generate a composition spec from a source file.").map {
      case (sourceFormat, targetFormat, write) -> files => CommandADT.Generate(files, sourceFormat, targetFormat, write)
    },
  )

  val app: CliApp[CommandExecutor, Nothing, CommandADT] = CliApp
    .make("tailcall", tailcall.BuildInfo.version.replace("v", ""), command.helpDoc.getSpan, command)(
      CommandExecutor.execute(_)
    ).summary(HelpDoc.Span.Text("Tailcall CLI"))
}
