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
    "replicas": {{spec.replicas}},
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
            "name": "compute-data-unit",
            "image": "jz-action/compute_data_runner:latest",
            "command": [
              "/compute_data_runner"
            ],
            "imagePullPolicy":"IfNotPresent",
            "ports": [
              {
                "containerPort": 80
              }
            ],
            "volumeMounts": [
              {
                "mountPath": "/unix_socket",
                "name": "unix-socket"
              },
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          },
          {
            "name": "compute-user-unit",
            "image": "{{{spec.image}}}",
            "command": [
              "sleep"
            ],
            "args": [
              "infinity"
            ],
            "volumeMounts": [
              {
                "mountPath": "/unix_socket",
                "name": "unix-socket"
              },
              {
                "mountPath": "/app/tmp",
                "name": "tmpstore"
              }
            ]
          }
        ],
        "volumes": [
          {
            "name": "unix-socket",
            "emptyDir": {}
          },
          {
            "name": "tmpstore",
            "persistentVolumeClaim": {
              "claimName":"{{name}}-node-claim"
            }
          }
        ]
      }
    }
  }
}