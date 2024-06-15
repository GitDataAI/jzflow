{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
    "name": "{{{name}}}-service",
    "exec_type": "compute_unit"
  },
  "spec": {
    "selector": {
      "app": "{{{name}}}-pod"
    },
    "ports": [
      {
        "name": "http",
        "protocol": "TCP",
        "port": 80,
        "targetPort": 80
      },
    ]
  }
}
