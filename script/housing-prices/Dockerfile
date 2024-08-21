FROM civisanalytics/datascience-python
RUN pip install seaborn  -i https://pypi.tuna.tsinghua.edu.cn/simple
RUN pip install requests-unixsocket2==0.4.1 -i https://pypi.tuna.tsinghua.edu.cn/simple
RUN pip install requests==2.32.3  -i https://pypi.tuna.tsinghua.edu.cn/simple 

WORKDIR /workdir/
COPY *.py /workdir/