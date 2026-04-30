# Contributing to AlertView

Thank you for your interest in contributing to AlertView! This guide covers how to contribute, the development process, and community guidelines.

## Ways to Contribute

There are many ways to contribute to AlertView:

### Code Contributions

- **New features**: Implement requested features from the issue tracker
- **Bug fixes**: Fix reported bugs
- **Performance improvements**: Optimize slow code paths
- **Refactoring**: Improve code quality and maintainability
- **Tests**: Add missing tests or improve test coverage
- **Documentation**: Improve or add documentation

### Non-Code Contributions

- **Report bugs**: Open issues for bugs you find
- **Request features**: Suggest new features or improvements
- **Review PRs**: Review open pull requests
- **Improve documentation**: Fix typos, improve clarity
- **Write tutorials**: Share your experience with AlertView
- **Help others**: Answer questions in discussions

### Community Contributions

- **Spread the word**: Share AlertView with others
- **Write blog posts**: Blog about your experience with AlertView
- **Give talks**: Present AlertView at meetups or conferences
- **Create integrations**: Build integrations with other tools

## Getting Started

### Prerequisites

1. **GitHub account**: Needed to open issues and pull requests
2. **Git**: For cloning and contributing to the repository
3. **Rust**: AlertView is written in Rust (1.75+)
4. **Docker** (optional): For testing Docker builds
5. **Kubernetes** (optional): For testing Kubernetes manifests

### Setup

1. **Fork the repository**:
   
   Go to [AlertView on GitHub](https://github.com/your-org/alertview) and click "Fork"

2. **Clone your fork**:

```bash
git clone https://github.com/your-username/alertview.git
cd alertview
```

3. **Add the upstream remote**:

```bash
git remote add upstream https://github.com/your-org/alertview.git
```

4. **Install Rust**:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

5. **Build and test**:

```bash
cargo build
cargo test
```

## Development Workflow

### Find Something to Work On

1. **Check open issues**: Look at the [issue tracker](https://github.com/your-org/alertview/issues)
2. **Look for "good first issue"**: These are marked as beginner-friendly
3. **Check the roadmap**: See what's planned for future releases
4. **Ask for suggestions**: Open a discussion or ask in chat

### Create a Branch

```bash
# Update your local repository
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git checkout -b feature/my-feature
# or for a bug fix
git checkout -b fix/my-bug-fix
```

**Branch naming conventions:**

| Type | Prefix | Example |
|------|--------|---------|
| Feature | `feature/` | `feature/alert-grouping` |
| Bug fix | `fix/` | `fix/config-reload` |
| Documentation | `docs/` | `docs/api-reference` |
| Refactoring | `refactor/` | `refactor/cache-module` |
| Chore | `chore/` | `chore/update-deps` |

### Make Your Changes

1. **Follow coding standards**: See [Structure](structure.md) and [Building](building.md)
2. **Write tests**: Add tests for new functionality
3. **Update documentation**: Update docs for any changes
4. **Keep commits atomic**: Each commit should be a single logical change

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/) for commit messages:

```
<type>(<scope>): <description>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring (no functional changes)
- `perf`: Performance improvements
- `test`: Adding or fixing tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes
- `build`: Build system changes

**Examples:**

```bash
# Good commit messages
git commit -m "feat(alerts): add alert grouping by labels"
git commit -m "fix(config): handle missing timeout field"
git commit -m "docs: update README with new features"
git commit -m "test(alerts): add tests for link templates"
git commit -m "refactor(cache): improve error handling"

# Bad commit messages (avoid these)
git commit -m "fixed stuff"
git commit -m "WIP"
git commit -m "update"
git commit -m "oops"
```

### Write Good Commit Messages

1. **Use the imperative mood**: "Add feature" not "Added feature" or "Adds feature"
2. **Capitalize the first letter**: "Fix bug" not "fix bug"
3. **No period at the end**: "Add feature" not "Add feature."
4. **Keep the first line under 50 characters**: For readability in git logs
5. **Separate subject from body with a blank line**: If you need a body
6. **Wrap the body at 72 characters**: Standard git convention
7. **Use the body to explain what and why**: Not how (the code shows how)

**Good example:**

```
feat(alerts): add support for Zabbix alerts

Add a new source kind 'zabbix' that fetches alerts from Zabbix API.
This allows users to monitor Zabbix alerts alongside Alertmanager
and Grafana alerts in a single dashboard.

Closes #123
```

### Push Your Changes

```bash
# Push to your fork
git push origin feature/my-feature
```

## Opening a Pull Request

### Before Opening a PR

1. **Run all checks**:

```bash
# Build
cargo build

# Run tests
cargo test

# Run linter
cargo clippy

# Check formatting
cargo fmt --check

# Run security audit
cargo audit
```

2. **Update documentation**: If your changes affect users
3. **Update CHANGELOG**: Add an entry for user-facing changes
4. **Squash commits**: If you have many small commits, squash them

### Creating the PR

1. Go to [AlertView Pull Requests](https://github.com/your-org/alertview/pulls)
2. Click "New pull request"
3. Select your fork and branch
4. Select the upstream main branch
5. Fill in the PR template
6. Click "Create pull request"

### PR Template

```markdown
## Description

<!-- Describe what this PR does -->

## Related Issues

<!-- Link to any related issues -->
- Closes #123
- Related to #456

## Changes

<!-- List the main changes -->
- Added feature X
- Fixed bug Y
- Updated documentation for Z

## Testing

<!-- Describe how you tested your changes -->
- [ ] All existing tests pass
- [ ] Added new tests for the changes
- [ ] Manually tested the feature
- [ ] Tested with Alertmanager
- [ ] Tested with Grafana
- [ ] Tested with Zabbix

## Checklist

- [ ] Code follows the project's style guidelines
- [ ] All tests pass
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation is updated
- [ ] CHANGELOG is updated (if applicable)
```

### PR Title

Use a clear, descriptive title:

```
# Good titles
feat: add alert grouping functionality
fix: prevent panic on invalid config
refactor: extract cache logic into separate module

# Bad titles (avoid these)
WIP: working on stuff
Fix bug
Update
```

## Review Process

### What to Expect

1. **Automated checks**: GitHub Actions will run tests, linter, etc.
2. **Initial review**: A maintainer will review your PR within a few days
3. **Feedback**: You may receive requests for changes
4. **Iteration**: Address feedback and push new commits
5. **Approval**: Once approved, a maintainer will merge your PR

### Addressing Feedback

1. **Be responsive**: Reply to comments in a timely manner
2. **Be open to suggestions**: Maintainers may suggest different approaches
3. **Ask for clarification**: If feedback is unclear, ask for more details
4. **Update the PR**: Push new commits to address feedback
5. **Request re-review**: After making changes, request another review

### Common Review Comments

| Comment | What it means | How to fix |
|---------|---------------|------------|
| "Please add tests" | Missing test coverage | Add unit/integration tests |
| "This could be extracted" | Code could be more modular | Extract into a function/module |
| "Consider using X" | There's a better way | Use the suggested approach |
| "This is hard to understand" | Code needs better documentation | Add comments, improve names |
| "This might panic" | Potential runtime error | Add error handling |
| "This is inefficient" | Performance concern | Optimize the code |

## Code Review Guidelines

### For Contributors

1. **Be patient**: Reviews take time
2. **Be open to feedback**: Don't take criticism personally
3. **Ask questions**: If you don't understand feedback, ask
4. **Keep PRs focused**: Smaller PRs are easier to review
5. **Follow up**: If a PR is stale, ping the reviewers

### For Reviewers

1. **Be kind**: Remember we're all learning
2. **Be specific**: Point to exact lines and explain issues
3. **Suggest improvements**: Don't just say "this is wrong"
4. **Be timely**: Try to review within a few days
5. **Focus on important issues**: Don't nitpick minor style issues

## Testing Your Changes

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --lib

# Run with verbose output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Test the API manually
cargo run
# Then visit http://localhost:8080
```

### Manual Testing

1. **Test with different sources**: Alertmanager, Grafana, Zabbix
2. **Test with different configurations**: Various config options
3. **Test edge cases**: Empty responses, errors, timeouts
4. **Test UI**: Verify the frontend works correctly

## Documentation

### When to Update Documentation

Update documentation when:
- Adding new features
- Changing existing behavior
- Fixing bugs that affected users
- Adding new configuration options
- Changing API endpoints

### Documentation Locations

| Type | Location |
|------|----------|
| User documentation | `docs/` |
| API documentation | `docs/api.md` |
| Configuration | `docs/configuration/` |
| Development docs | `docs/development/` |
| README | `README.md` |
| Code comments | In the source code |

### Documentation Style

- Use Markdown for all documentation
- Keep sentences short and clear
- Use code examples where helpful
- Include screenshots for UI changes (if applicable)
- Use consistent formatting

## Community Guidelines

### Code of Conduct

AlertView follows a Code of Conduct to ensure a welcoming community. By participating, you agree to abide by this code.

**Our Pledge:**

We pledge to make participation in our community a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, sex characteristics, gender identity and expression, level of experience, education, socio-economic status, nationality, personal appearance, race, religion, or sexual identity and orientation.

**Our Standards:**

Examples of behavior that contributes to creating a positive environment include:
- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

Examples of unacceptable behavior by participants include:
- The use of sexualized language or imagery
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate in a professional setting

**Enforcement:**

Violations of the Code of Conduct may result in a temporary or permanent ban from the community.

### Communication

| Channel | Purpose | Response Time |
|---------|---------|---------------|
| GitHub Issues | Bug reports, feature requests | Within a few days |
| GitHub Discussions | General questions, ideas | Within a week |
| GitHub PRs | Code contributions | Within a few days |
| Email | Private matters | Varies |

### Reporting Issues

When reporting a bug or requesting a feature:

1. **Search existing issues**: Check if it's already been reported
2. **Use a clear title**: Describe the issue briefly
3. **Provide details**: Steps to reproduce, expected vs actual behavior
4. **Include version**: AlertView version, Rust version, OS
5. **Add labels**: Use appropriate labels (bug, enhancement, etc.)

**Good issue example:**

```markdown
## Description

When I try to load a configuration with an invalid timeout value, AlertView panics instead of returning an error.

## Steps to Reproduce

1. Create a config file with:
   ```yaml
   sources:
     - name: test
       type: alertmanager
       url: http://localhost:9093
       timeout: -1
   ```
2. Run AlertView with this config
3. Observe the panic

## Expected Behavior

AlertView should return an error message about the invalid timeout value.

## Actual Behavior

AlertView panics with:
```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ...
```

## Environment

- AlertView version: v0.1.0
- Rust version: 1.75.0
- OS: Ubuntu 22.04
```

**Bad issue example (avoid this):**

```markdown
It doesn't work

Help!
```

## Versioning

AlertView follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes, incompatible API changes
- **MINOR**: New features, backward-compatible changes
- **PATCH**: Bug fixes, backward-compatible changes

### When to Bump Version

| Change | Version Bump |
|--------|--------------|
| Breaking API changes | MAJOR |
| New features | MINOR |
| Bug fixes | PATCH |
| Documentation changes | None (or PATCH if significant) |
| Internal refactoring | None |

## Releases

### Release Process

1. **Update version**: In `Cargo.toml` and any other files
2. **Update CHANGELOG**: Add entries for all changes since last release
3. **Create Git tag**: `git tag -a v1.0.0 -m "Release v1.0.0"`
4. **Push tag**: `git push origin v1.0.0`
5. **Create GitHub release**: With release notes from CHANGELOG
6. **Publish crate**: `cargo publish` (if applicable)
7. **Update Docker image**: Build and push new Docker image

### Release Checklist

- [ ] All tests pass
- [ ] All clippy warnings are addressed
- [ ] Code is formatted
- [ ] Documentation is complete
- [ ] CHANGELOG is updated
- [ ] Version is updated in all files
- [ ] Git tag is created and pushed
- [ ] GitHub release is created
- [ ] Docker image is built and pushed
- [ ] Package managers are updated (if applicable)

## Maintenance

### Issue Triage

Help triage issues by:
- Reproducing reported bugs
- Asking for more information
- Suggesting workarounds
- Labeling issues appropriately
- Closing stale or duplicate issues

### PR Triage

Help triage PRs by:
- Reviewing open PRs
- Testing changes locally
- Providing feedback
- Requesting changes when needed
- Approving ready PRs

### Documentation Maintenance

Help maintain documentation by:
- Updating docs for new features
- Fixing typos and errors
- Improving clarity
- Adding examples
- Translating to other languages

## Recognition

### Contributor Recognition

All contributors are recognized in:
- The [CONTRIBUTORS.md](CONTRIBUTORS.md) file
- GitHub's contributor graph
- Release notes (for significant contributions)

### Becoming a Maintainer

Active contributors may be invited to become maintainers. Maintainers have:
- Write access to the repository
- Ability to merge PRs
- Ability to manage issues and PRs
- Responsibility for project direction

**Maintainer responsibilities:**
- Review PRs in a timely manner
- Triage issues
- Guide the project's direction
- Ensure code quality
- Make releases
- Engage with the community

## Additional Resources

- [GitHub Flow](https://docs.github.com/en/get-started/quickstart/github-flow): GitHub's recommended workflow
- [Conventional Commits](https://www.conventionalcommits.org/): Commit message conventions
- [Semantic Versioning](https://semver.org/): Versioning guidelines
- [Rust Documentation](https://www.rust-lang.org/learn): Learn Rust
- [Git Documentation](https://git-scm.com/doc): Learn Git

## FAQ

### How do I start contributing?

1. Look at open issues, especially those labeled "good first issue"
2. Ask in discussions if you need help finding something to work on
3. Start small with documentation or simple bug fixes

### Do I need to be an expert in Rust?

No! We welcome contributors at all skill levels. Start with what you know and learn as you go.

### How long does it take to get a PR reviewed?

We try to review PRs within a few days. If your PR hasn't been reviewed in a week, feel free to ping the maintainers.

### What if my PR isn't accepted?

Don't be discouraged! We'll provide feedback on why it wasn't accepted and how you can improve it.

### Can I work on multiple things at once?

Yes! But we recommend focusing on one thing at a time, especially when you're getting started.

### How do I get help?

- Open a discussion on GitHub
- Ask in the issue tracker
- Reach out to maintainers directly (but prefer public channels)

### Can I contribute without writing code?

Absolutely! Documentation, testing, triage, and community support are all valuable contributions.
