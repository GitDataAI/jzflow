{
  "apiVersion": "apps/v1",
  "kind": "Deployment",
  "metadata": {
    "name": "{{{name}}}-channel-deployment",
    "id": "{{{id}}}",
    "labels": {
      "exec-type": "channel"
    }
  },
  "spec": {
    "replicas": {{{channel.spec.replicas}}},
    "selector": {
      "matchLabels": {
        "app": "{{{name}}}-channel-pod"
      }
    },
    "template": {
      "metadata": {
        "labels": {
          "app": "{{{name}}}-channel-pod"
        }
      },
      "spec": {
        "containers": [
          {
            "name": "channel",
            "image": "{{{channel.spec.image}}}",
            "command": [ "sleep" ],
            "args": [ "infinity" ],
            "ports": [
              {
                "containerPort": 90
              }
            ]
          }
        ]
      }
    }
  }
}
