# Submitting Game Data

The **ps5rs** project stores game metadata as JSON files.  When you have a new game (or a new version of an existing game) you can contribute it to the public dataset.

---

## 1. Fork & clone the repository
```sh
# on GitHub
git clone https://github.com/your‑username/ps5rs.git
cd ps5rs
```

## 2. Add the JSON files
* **Where to put them**
  * For a *single‑game* dataset add the file under `load/games/` with the naming pattern:
    ```
    load/games/<GameTitle>_-[<PPSAxxxxx>].json
    ```
    Example: `load/games/Blue_Prince_-_[PPSA25009].json`
  * For a *full analysis* (including per‑version inventory, middleware, shader, etc.) add the directory tree under `analysis_with_modules/` exactly as the other games are stored.  The top‑level directory must be `games/<PPSAxxxxx>/versions/<hash>/` and contain the JSON artifacts (`game.json`, `inventory_slice.json`, …).  You can copy the existing folder of a similar game and replace the contents.

* **Naming convention**
  * The bracketed PPSA ID **must** be present and unique (`[PPSA#####]`).
  * Use only ASCII characters in the filename.

## 3. Validate locally
Run the full test suite to ensure the new data does not break anything:
```sh
cargo test -p ps5-analysis   # runs the scanner & validation tests
cargo test -p ps5-emu        # runs the emulator fixture suite
```
All tests should pass.  If you added a new version under `analysis_with_modules/`, the `ps5-analysis` tests will automatically pick it up.

## 4. Commit & push
```sh
git add load/games/<your‑file>.json   # or the new analysis directory
git commit -m "data: add <GameTitle> – PPSAxxxxx"
git push origin HEAD
```

## 5. Open a Pull Request
* Go to the **ps5rs** upstream repository on GitHub.
* Click **"New pull request"**, choose your branch, and write a short description:
  * What game was added.
  * Any notable differences (e.g., custom patches, missing modules).
* The CI will run `cargo test` for all crates; the PR can be merged once the pipeline is green.

---

### Tips & Gotchas
* **Do not modify existing game data** unless you are fixing a mistake – changes to historic games require a justification comment in the PR.
* Keep the JSON files **compact** – extra whitespace is fine, but do not include binary blobs.  All binary information is derived from the ELF fixtures.
* If you need to generate new ELF fixtures, use the existing generator:
  ```sh
  cargo run -p ps5-tests --bin generate
  ```
  This will update `data/test/generated_elfs/` and can be copied into the appropriate analysis directory.

---

**Thank you for helping improve the ps5rs dataset!**
