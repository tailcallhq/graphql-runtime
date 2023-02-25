package tailcall.gateway.service

import caliban.introspection.adt.__Type
import caliban.parsing.SourceMapper
import caliban.parsing.adt.Definition.TypeSystemDefinition.TypeDefinition.{FieldDefinition, InputValueDefinition}
import caliban.parsing.adt.{Definition => CalibanDefinition, Document => CalibanDocument, Type => CalibanType}
import caliban.tools.RemoteSchema.parseRemoteSchema
import tailcall.gateway.ast.Document
import tailcall.gateway.internal.DynamicValueUtil
import zio.{ZIO, ZLayer}

trait DocumentTypeGenerator {
  def __type(doc: Document): __Type
}

object DocumentTypeGenerator {
  def __type(document: Document): ZIO[DocumentTypeGenerator, Nothing, __Type] =
    ZIO.serviceWith[DocumentTypeGenerator](_.__type(document))

  def live: ZLayer[Any, Nothing, DocumentTypeGenerator] = ZLayer.succeed(new Live())

  final class Live extends DocumentTypeGenerator {
    // TODO: fix this implementation
    override def __type(doc: Document): __Type =
      parseRemoteSchema(toCalibanDocument(doc)).map(_.queryType).getOrElse(???)

    private def toCalibanDocument(document: Document): CalibanDocument = {
      CalibanDocument(
        document.definition.map {
          case Document.ObjectTypeDefinition(name, fields)      => CalibanDefinition.TypeSystemDefinition.TypeDefinition
              .ObjectTypeDefinition(None, name, Nil, Nil, fields.map(toCalibanField))
          case Document.InputObjectTypeDefinition(name, fields) => CalibanDefinition.TypeSystemDefinition.TypeDefinition
              .InputObjectTypeDefinition(None, name, Nil, fields.map(toCalibanInputValue))
          case Document.SchemaDefinition(queries, mutations, subscriptions) => CalibanDefinition.TypeSystemDefinition
              .SchemaDefinition(Nil, queries, mutations, subscriptions)
        },
        SourceMapper.empty
      )
    }

    private def toCalibanField(field: Document.FieldDefinition): FieldDefinition =
      FieldDefinition(None, field.name, field.args.map(toCalibanInputValue), toCalibanType(field.ofType), Nil)

    private def toCalibanInputValue(inputValue: Document.InputValueDefinition): InputValueDefinition =
      CalibanDefinition.TypeSystemDefinition.TypeDefinition.InputValueDefinition(
        None,
        inputValue.name,
        toCalibanType(inputValue.ofType),
        inputValue.defaultValue.map(DynamicValueUtil.toInputValue),
        Nil
      )

    private def toCalibanType(tpe: Document.Type): CalibanType =
      tpe match {
        case Document.NamedType(name, nonNull)  => CalibanType.NamedType(name, nonNull)
        case Document.ListType(ofType, nonNull) => CalibanType.ListType(toCalibanType(ofType), nonNull)
      }
  }
}
