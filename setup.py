
from setuptools import setup

setup(
   name='jzflowsdk',
   version='0.1',
   description='simple sdk function for write jzflow node',
   author='gitdatateam',
   author_email='',
   packages=['jzflowsdk'],
   package_dir={'jzflowsdk':'crates/jzflowsdk_py'},
   install_requires=[
        'requests-unixsocket2==0.4.1',
        'requests==2.32.3',
    ]
)