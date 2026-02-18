# CLAUDE.md

## Testing

```bash
python3 test_safe_chains.py
```

All tests must pass before committing.

## Development

- The hook is Python 3, stdlib only, no external dependencies
- When adding a new command handler: add the handler in `safe-chains.sh`, add test cases in `test_safe_chains.py` covering both allow and deny, run the test suite
- Do not add comments to code
- All files must end with a newline
