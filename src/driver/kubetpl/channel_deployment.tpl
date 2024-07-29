{
  "apiVersion": "apps/v1",
  "kind": "Deployment",
  "metadata": {
    "name": "{{{node.name}}}-channel-deployment",
    "id": "{{{id}}}",
    "labels": {
      "exec-type": "channel"
    }
  },
  "spec": {
    "replicas": {{{node.channel.spec.replicas}}},
    "selector": {
      "matchLabels": {
        "app": "{{{node.name}}}-channel-pod"
      }
    },
    "template": {
      "metadata": {
        "labels": {
          "app": "{{{node.name}}}-channel-pod"
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
            "args":[
              "--node-name={{{node.name}}}",
              "--log-level={{{log_level}}}",
              "--mongo-url={{{db.mongo_url}}}",
              "--database={{{db.database}}}"
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
              "claimName":"{{node.name}}-channel-claim"
            }
          }
        ]
      }
    }
  }
}
