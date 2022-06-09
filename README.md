```
railyard ---> boxcar-cluster ---> boxcar-rpc
```

1. Load Branch step object
2. Branch eval calls `BoxcarJob.eval()`
3. BoxcarJob checks the status of the boxcar call

How do we track Step state? 

```json
{
  "step_eval_id": "<uuid>",
  "data": {},
  "children": [
    
  ]
}
```