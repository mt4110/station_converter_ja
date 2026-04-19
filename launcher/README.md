# launcher

`launcher/quickstart_tui.py` is the local quickstart console for this repo.

```bash
python3 launcher/quickstart_tui.py
```

What it gives you:

- one-shot `Quick Start`
- individual control for prepare / migrate / ingest / validate
- status for PostgreSQL / MySQL / DB Web / API / Sample Web / crawler loop
- quick path tips that show the fastest order to follow
- recent logs and last-run timestamps for the selected item
- current workflow step and elapsed time while Quick Start is running
- browser jump points for Sample Web, API ready, and DB Web

Useful keys:

- first selectable rows are `Language -> Japanese / English`
- next rows are `Validate Mode -> Standard / Strict`
- `1` / `2` / `3`: switch DB mode (`postgres` / `mysql` / `sqlite`)
- `v`: toggle validate mode (`standard` / `strict`)
- top row `Language: Japanese / English` shows the current choice
- `l` or `Tab`: toggle English / Japanese
- `Enter` or `s`: run / start selected item
- `x`: stop selected item, or cancel the running workflow
- `r`: restart selected service
- `o`: open the selected URL
- `q`: quit

Text snapshot without curses:

```bash
python3 launcher/quickstart_tui.py --status
```

The behavior lives in [`launcher/station_converter_ja.json`](./station_converter_ja.json), so the same TUI can be reused in sibling repos such as `postal_converter_ja` by swapping the config file.

The supported runtime entry points remain explicit:

- `station-api`
- `station-ops job ingest-n02`
- `station-ops export-sqlite`
