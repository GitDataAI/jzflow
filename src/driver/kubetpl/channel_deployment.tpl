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
            "image": "jz-action/dp_runner:latest",
            "imagePullPolicy":"IfNotPresent",
            "command": [
              "/dp_runner"
            ],
            "ports": [
              {
                "containerPort": 80
              }
            ],
            "volumeMounts": [
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          }
        ],
        "volumes": [
             {
            "name": "tmpstore",
            "persistentVolumeClaim": {
              "claimName":"{{name}}-channel-claim"
            }
          }
        ]
      }
    }
  }
}
