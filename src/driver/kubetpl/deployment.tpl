{
  "apiVersion": "apps/v1",
  "kind": "Deployment",
  "metadata": {
    "name": "{{{name}}}-deployment",
    "id": "{{{id}}}",
    "labels": {
      "exec_type": "compute_unit"
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
            "name": "compute_unit",
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
