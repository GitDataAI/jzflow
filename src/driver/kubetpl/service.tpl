{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
    "name": "{{{name}}}-service",
    "exec-type": "compute-unit"
  },
  "spec": {
    "selector": {
      "app": "{{{name}}}-pod"
    },
    "ports": [
      {
        "port": 80,
        "targetPort": 80
      }
    ]
  }
}
