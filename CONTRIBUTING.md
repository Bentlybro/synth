# Contributing to Synth

Thanks for your interest in contributing to Synth! This document provides guidelines for contributing.

## Code of Conduct

- Be respectful and constructive
- Focus on the problem, not the person
- Assume good intentions

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in Issues
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - Your environment (OS, Rust version, etc.)
   - Relevant logs

### Suggesting Features

1. Check if the feature has already been suggested
2. Create an issue describing:
   - The problem it solves
   - Your proposed solution
   - Alternative approaches considered
   - Why this would be useful

### Pull Requests

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Ensure code compiles: `cargo build --release`
4. Run tests if available: `cargo test`
5. Format code: `cargo fmt`
6. Run linter: `cargo clippy`
7. Update documentation if needed
8. Commit with a clear message describing your changes
9. Push to your fork and create a pull request

**PR Guidelines:**
- Keep changes focused (one feature/fix per PR)
- Write clear commit messages
- Update README if adding features
- Add tests for new functionality when possible

### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/synth.git
cd synth

# Copy environment template
cp .env.example .env

# Edit .env with your API keys
# ANTHROPIC_API_KEY=your_key_here
# OPENAI_API_KEY=your_key_here (optional, for YouTube)

# Install SearXNG (required)
# See README.md for installation instructions

# Build and run
cargo build --release
./target/release/synth
```

## Priority Areas

We'd especially welcome contributions in:

- **Performance**: Optimize scraping, caching, or LLM calls
- **Features**: PDF scraping, streaming responses, timing metrics
- **Documentation**: Better examples, guides, tutorials
- **Testing**: Unit tests, integration tests, benchmarks
- **Deployment**: Docker improvements, Kubernetes configs
- **Multi-LLM**: Support for local Ollama, OpenAI fallback

## Questions?

Feel free to open an issue labeled "question" or reach out to the maintainers.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
