import pipeline.fastly.kubernetes.jenkins.PodTemplates

def containers = []
containers << [dockerFile: 'Dockerfile', dockerContextPath: '.', imageName: 'fastly/isolation', timeout: 1200]
def builtTag = fastlyDockerBuild([script: this, containers: containers])

def podTemplates = new PodTemplates(this)

def builders = [:]
builders['fastly/isolation-test'] = {
  podTemplates.adhocImage('fastly/isolation', builtTag) {
    checkout scm
    sh "make audit indent-check test"
  }
}

builders['fastly/isolation-bench'] = {
  podTemplates.adhocImage('fastly/isolation', builtTag) {
    checkout scm
    sh "make bench"
  }
}

parallel builders
