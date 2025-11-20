# Contributing to SLIMS REST API

We welcome contributions from the community to make the SLIMS REST API even better! Whether you're fixing bugs, adding new features, improving documentation, or suggesting enhancements, your help is greatly appreciated.

## How to Contribute

The contribution workflow generally follows these steps:

1.  **Fork the Repository:** Start by forking the official `slims-rest-api` repository to your GitHub account.
2.  **Clone Your Fork:** Clone your forked repository to your local machine:
    ```bash
    git clone https://github.com/your-username/slims-rest-api.git
    cd slims-rest-api
    ```
3.  **Create a New Branch:** Create a new branch for your feature or bug fix. Use a descriptive name (e.g., `feature/add-member-search`, `fix/login-bug`).
    ```bash
    git checkout -b feature/your-feature-name
    ```
4.  **Make Your Changes:** Implement your feature or fix the bug. Ensure your code adheres to the existing coding style and conventions.
    *   **Run Tests:** Before committing, always run the existing test suite to ensure your changes haven't introduced any regressions. (Note: The project currently lacks a dedicated test suite; however, if one is added in the future, it should be run here.)
    *   **Write Tests:** For new features or bug fixes, please write appropriate unit and/or integration tests to cover your changes.
5.  **Commit Your Changes:** Write clear and concise commit messages. A good commit message explains *what* was changed and *why*.
    ```bash
    git commit -m "feat: Add member search functionality"
    ```
6.  **Push to Your Fork:** Push your new branch to your forked repository on GitHub:
    ```bash
    git push origin feature/your-feature-name
    ```
7.  **Create a Pull Request (PR):**
    *   Go to the original `slims-rest-api` repository on GitHub.
    *   You should see a prompt to create a new Pull Request from your recently pushed branch.
    *   Provide a clear title and detailed description for your PR. Explain the problem it solves or the feature it adds, and any relevant technical details.
    *   Reference any related issues (e.g., `Fixes #123`, `Closes #456`).
8.  **Review Process:** Your PR will be reviewed by the maintainers. Be prepared to discuss your changes and make any necessary adjustments based on feedback.
9.  **Merge:** Once approved, your changes will be merged into the main branch!

## Coding Style and Standards

*   **Rustfmt:** The project uses `rustfmt` for code formatting. Please ensure your code is formatted correctly before submitting:
    ```bash
    cargo fmt
    ```
*   **Clippy:** We use `clippy` for linting. Please address any warnings:
    ```bash
    cargo clippy
    ```
*   **Asynchronous Rust:** Adhere to idiomatic asynchronous Rust patterns, especially with `Actix-web` and `SQLx`.
*   **JSON:API Compliance:** All new API endpoints and modifications to existing ones must strictly follow the [JSON:API specification](https://jsonapi.org/).

## Reporting Bugs

If you find a bug, please open an issue on the GitHub repository. Provide as much detail as possible, including:

*   A clear and concise description of the bug.
*   Steps to reproduce the behavior.
*   Expected behavior.
*   Actual behavior.
*   Any relevant error messages or logs.
*   Your environment (OS, Rust version, database version).

## Suggesting Enhancements

We're always looking for ways to improve! If you have an idea for a new feature or an improvement to an existing one, please open an issue to discuss it.

Thank you for your contributions!
