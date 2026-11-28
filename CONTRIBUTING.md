# Contributing to fotobuch

Thank you for considering a contribution. Please read this document before
opening a pull request — in particular the [Use of AI](#use-of-ai) policy and
the [Contribution Terms](#contribution-terms), which every contribution is
subject to.

## Use of AI

This policy applies to external contributors.

Using AI (i.e., LLMs) as a tool for coding is welcome. However, a relative high bar is
held for all contributions: you remain responsible for any code you submit, and
the maintainer remains responsible for any code that is merged and released.

- **You must understand most of the change you submit.** You are expected to be able
  to explain the proposed changes in your own words — in the pull request body
  and when answering questions. Pull requests containing LLM-generated work the
  author does not understand at all will be closed. This does not mean that you
  need to understand each and every line, but in this case be open about it.
- **Comments must be written by humans.** Do not use AI to generate review replies or issue comments,
  and do not simply copy AI responses when answering questions from the maintainer. Comments believed to
  be AI-generated may be hidden without notice.
- **No autonomous agents.** A human in the loop is required. Pull requests that
  appear to be maintained autonomously will be closed, possibly without notice.
- **Disclose AI context.** If you want to share context from an interaction
  with an AI, put the relevant part in a quote block (`>`), mark it as
  AI-generated, and add your own commentary on why it is relevant. Do not share
  long snippets.
- **PR descriptions are fine** You can use AI to generate an PR description,
  as long as you understand every part of it fully and consider it concise enough and NOT bloated.
- **Translations are fine.** If you use AI to translate your comments, write in
  your native language and include the AI translation in a quote block
  beneath it.

This policy was adapted from
[ripgrep's AI policy](https://github.com/BurntSushi/ripgrep/blob/master/AI_POLICY.md).

## Workflow

- Use the terms defined in the [glossary](docs/book/src/glossary.md) in code,
  docs and commit messages.
- Every commit follows [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat: …`, `fix: …`, `docs: …`, …).
- New features come with unit tests.
- Page indices are 0-based everywhere (cover = 0), internally and in the CLI.
- Before pushing, make sure the following pass without warnings:

  ```bash
  cargo fmt --check
  cargo clippy --all-features --all-targets -- -D warnings
  cargo test
  ```

- New dependencies must use a license accepted in [`deny.toml`](deny.toml);
  `cargo deny check licenses` verifies this in CI.

## Contribution Terms

By submitting a contribution to this project (a pull request, patch, or any
other material, hereinafter "Contribution"), you agree to the following terms:

1. **Copyright license.** You license your Contribution to everyone under the
   MIT No Attribution License (MIT-0), reproduced below. You keep the
   copyright in your Contribution. The project as a whole is distributed
   under the GNU Affero General Public License v3.0 or later; the maintainer
   may additionally distribute your Contribution, as part of the project or
   otherwise, under any other license, including commercial licenses.

2. **Patent license.** You grant everyone a perpetual, worldwide,
   non-exclusive, no-charge, royalty-free, irrevocable patent license to
   make, have made, use, offer to sell, sell, import, and otherwise transfer
   your Contribution, alone or in combination with the project, for every
   patent claim licensable by you that is necessarily infringed by your
   Contribution alone or by its combination with the project.

3. **Your right to contribute.** You confirm that the Contribution is your
   own original work, or that you otherwise have the right to submit it
   under these terms (for example, your employer has agreed). If the
   Contribution contains third-party material, you identify that material
   and its license in the pull request.

4. **No obligation.** The maintainer is not obliged to accept or use your
   Contribution.

You confirm your agreement by ticking the corresponding checkbox in the pull
request template.

### MIT No Attribution License

```text
MIT No Attribution

Permission is hereby granted, free of charge, to any person obtaining a copy of this
software and associated documentation files (the "Software"), to deal in the Software
without restriction, including without limitation the rights to use, copy, modify,
merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```
