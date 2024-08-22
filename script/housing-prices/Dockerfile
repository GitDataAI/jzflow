FROM civisanalytics/datascience-python
RUN pip install seaborn  -i https://pypi.tuna.tsinghua.edu.cn/simple
RUN pip install git+https://github.com/GitDataAI/jiaoziflow.git

WORKDIR /workdir/
COPY *.py /workdir/