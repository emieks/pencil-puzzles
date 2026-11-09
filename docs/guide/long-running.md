# Long-running calls

Generating large or expert puzzles can take a while. These options bound the
work, stop a call early and run calls in threads.

## Limits

Every function takes a `limits` argument:

```python
pp.generate("kakuro", (16, 16), difficulty="expert",
            limits=pp.Limits(work=10_000_000, attempts=100))
```

| Limit | Default | Bounds | Raises |
| --- | --- | --- | --- |
| `work` | 100,000,000 | The computation in one call | `BudgetExceededError` |
| `attempts` | 10,000 | The candidate puzzles `generate` tries | `AttemptsExhaustedError` |

A result never depends on the limits: a call either returns the same result
as with higher limits, or raises.

## Cancellation

To stop a call from another thread, pass a `pp.CancellationToken` as
`cancel` and call its `cancel()` method. The call raises `CancelledError`.

```python
import threading

token = pp.CancellationToken()
threading.Timer(5.0, token.cancel).start()
try:
    pp.generate("kakuro", (16, 16), difficulty="expert", cancel=token)
except pp.CancelledError:
    print("gave up after 5 seconds")
```

A token stays cancelled; use a new one for the next call. `token.cancelled`
tells whether it was cancelled.

## Threads

The engine runs without holding the GIL, so other Python threads keep
running during a call, and calls in separate threads run in parallel.
Ctrl-C during a call raises `KeyboardInterrupt`.
