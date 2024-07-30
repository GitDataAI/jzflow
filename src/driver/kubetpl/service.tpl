{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
    "name": "{{{name}}}-service",
    "exec-type": "compute-unit"
  },
  "spec": {
    clusterIP: None,
    "selector": {
      "app": "{{{name}}}-pod"
    },
    "ports": [
      {
        "name": "http",
        "protocol": "TCP",
        "port": 80,
        "targetPort": 80
      }
    ]
  }
}
