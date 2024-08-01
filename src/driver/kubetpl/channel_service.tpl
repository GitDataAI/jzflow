{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
      "name": "{{{node.name}}}-channel-headless-service",
      "exec-type": "channel"
  },
  "spec": {
    "clusterIP": "None",
    "selector": {
      "app": "{{{node.name}}}-channel-pod"
    },
    "ports": [
      {
        "port": 80,
        "targetPort": 80
      }
    ]
  }
}
