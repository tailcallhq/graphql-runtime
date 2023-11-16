# Contribution Guidelines

Thank you for considering contributing to **Tailcall**! This document outlines the steps and guidelines to follow when contributing to this project.

## Getting Started

1. **Fork the Repository:** Start by forking the repository to your personal account on GitHub.
2. **Clone the Forked Repository:** Once you have forked the repository, clone it to your local machine.
   ```bash
   git clone https://github.com/tailcallhq/tailcall.git
   ```

## Setting Up the Development Environment

1. **Install Rust:** If you haven't already, install Rust using [rustup](https://rustup.rs/). Install the `nightly` toolchain as well, as it's used for linting.
2. **Install Prettier:** Install [Prettier](https://prettier.io/) too as this is also used for linting.
3. **Build the Application:** Navigate to the project directory and build the application.

   ```bash
   cd tailcall
   cargo build
   ```

4. **Start the Server:** To start the server, use the following command:
   ```bash
   cargo run -- start ./examples/jsonplaceholder.graphql
   ```
   Once the server is running, you can access the GraphiQL interface at [http://localhost:8000/graphiql](http://localhost:8000/graphiql).

## Making Changes

1. **Create a New Branch:** Always create a new branch for your changes.

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Write Clean Code:** Ensure your code is clean, readable, and well-commented.
3. **Follow Rust Best Practices:** Adhere to the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html).
4. **Use Title Case in Job Names:** When adding new CI jobs to `.github/workflows`, please use title case e.g. _Close Stale Issues and PR_.

## Testing

1. **Write Tests:** For every new feature or bugfix, ensure that you write appropriate tests.
2. **Run Tests:** Before submitting a pull request, ensure all tests pass.
   ```bash
   cargo test
   ```

## Documentation

1. **Update README:** If your changes necessitate a change in the way users interact with the application, update the README accordingly.
2. **Inline Documentation:** Add inline documentation to your code where necessary.

## Committing Your Changes

1. **Atomic Commits:** Make sure each commit is atomic (i.e., it does one thing). This makes it easier to review and revert if necessary.
2. **Commit Message Guidelines:** Write meaningful commit messages. Start with a short summary (50 chars max), followed by a blank line and then a detailed description if needed.

## Submitting a Pull Request

1. **Push to Your Fork:** Push your changes to your fork on GitHub.

   ```bash
   git push origin feature/your-feature-name
   ```

2. **Open a Pull Request:** Navigate to the original repository on GitHub and open a pull request against the `main` or `develop` branch.
3. **Describe Your Changes:** In the pull request description, explain the changes you made, the issues they resolve, and any other relevant information.
4. **Wait for Review:** Maintainers will review your pull request. Address any comments or feedback they provide.

## Spread the Word

1. **Star the Repository:** If you find this project useful, please give it a star on GitHub. This helps increase its visibility and encourages more people to contribute.
2. **Tweet About Your Contribution:** Share your contributions and experiences with the wider community on Twitter. Use the hashtag `#TailcallContributor` and tag `@tailcallhq` to let us know!

## Community

1. **Be Respectful:** Please remember that this is an open-source project and the community is welcoming and respectful to all members.

## Final Words

Thank you for contributing! Your efforts help improve the application for everyone.
