{
  "apiVersion": "apps/v1",
  "kind": "Deployment",
  "metadata": {
    "name": "{{{name}}}-deployment",
    "id": "{{{id}}}",
    "labels": {
      "exec-type": "compute-unit"
    }
  },
  "spec": {
    "replicas": {{{spec.replicas}}},
    "selector": {
      "matchLabels": {
        "app": "{{{name}}}-pod"
      }
    },
    "template": {
      "metadata": {
        "labels": {
          "app": "{{{name}}}-pod"
        }
      },
      "spec": {
        "containers": [
          {
            "name": "compute-unit",
            "image": "{{{spec.image}}}",
            "command": [ "sleep" ],
            "args": [ "infinity" ],
            "ports": [
              {
                "containerPort": 80
              }
            ]
          }
        ]
      }
    }
  }
}
