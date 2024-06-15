{
  "apiVersion": "v1",
  "kind": "Service",
  "metadata": {
      "name": "{{{name}}}-channel-service",
      "exec_type": "channel"
  },
  "spec": {
    "selector": {
      "app": "{{{name}}}-channel-pod"
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
