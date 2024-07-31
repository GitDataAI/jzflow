{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
    "name": "{{{name}}}-headless-service",
    "exec-type": "compute-unit"
  },
  "spec": {
    "clusterIP": "None",
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
