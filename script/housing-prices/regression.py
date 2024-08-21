import argparse
import os
import time
from os import path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import joblib
import logging
import uuid
from utils import plot_learning_curve
from ipc import IPCClient, IPCError

from sklearn.model_selection import ShuffleSplit
from sklearn import datasets, ensemble, linear_model
from sklearn.model_selection import learning_curve
from sklearn.model_selection import ShuffleSplit
from sklearn.model_selection import cross_val_score

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

parser = argparse.ArgumentParser(description="Structured data regression")
parser.add_argument("--target-col",
                    type=str,
                    help="column with target values")

def load_data(input_csv, target_col):
    # Load the Boston housing dataset
    data = pd.read_csv(input_csv, header=0)
    targets = data[target_col]
    features = data.drop(target_col, axis = 1)
    print("Dataset has {} data points with {} variables each.".format(*data.shape))
    return data, features, targets

def create_pairplot(data):
    plt.clf()
    # Calculate and show pairplot
    sns.pairplot(data, height=2.5)
    plt.tight_layout()

def create_corr_matrix(data):
    plt.clf()
    # Calculate and show correlation matrix
    sns.set()
    corr = data.corr()
    
    # Generate a mask for the upper triangle
    mask = np.triu(np.ones_like(corr, dtype=np.bool))

    # Generate a custom diverging colormap
    cmap = sns.diverging_palette(220, 10, as_cmap=True)

    # Draw the heatmap with the mask and correct aspect ratio
    sns_plot = sns.heatmap(corr, mask=mask, cmap=cmap, vmax=.3, center=0,
                square=True, linewidths=.5, annot=True, cbar_kws={"shrink": .5})

def train_model(features, targets):
    # Train a Random Forest Regression model
    reg = ensemble.RandomForestRegressor(random_state=1)
    scores = cross_val_score(reg, features, targets, cv=10)
    print("Score: {:2f} (+/- {:2f})".format(scores.mean(), scores.std() * 2))
    reg.fit(features,targets)
    return reg

def create_learning_curve(estimator, features, targets):
    plt.clf()

    title = "Learning Curves (Random Forest Regressor)"
    cv = ShuffleSplit(n_splits=10, test_size=0.2, random_state=0)
    plot_learning_curve(estimator, title, features, targets, 
                        ylim=(0.5, 1.01), cv=cv, n_jobs=4)

def main():
    client = IPCClient()
    tmp_path = "/app/tmp"
    args = parser.parse_args()
    
    while True:
        instant = time.time()
        try:
            available_data = client.request_available_data()
            if available_data is not None:
                id = available_data['id']
                path_str = os.path.join(tmp_path, id)
                root_input_dir = path_str

                new_id = str(uuid.uuid4())
                output_dir = os.path.join(tmp_path, new_id, "images")
                os.makedirs(output_dir, exist_ok=True)

                for dirpath, dirs, files in os.walk(root_input_dir):  
                    input_files = [ os.path.join(dirpath, filename) for filename in files if filename.endswith('.csv') ]
                logger.info(f"Datasets: {input_files}")
                for filename in input_files:

                    experiment_name = os.path.basename(os.path.splitext(filename)[0])
                    # Data loading and Exploration
                    data, features, targets = load_data(filename, args.target_col)
                    create_pairplot(data)
                    plt.savefig(path.join(output_dir, experiment_name + '_pairplot.png'))
                    logger.info("save 1")
                    create_corr_matrix(data)
                    plt.savefig(path.join(output_dir, experiment_name + '_corr_matrix.png'))
                    logger.info("save 1")
                    # Fit model
                    reg = train_model(features, targets)
                    create_learning_curve(reg, features, targets)
                    plt.savefig(path.join(output_dir, experiment_name + '_cv_reg_output.png'))
                    logger.info("save 1")
                    # Save model
                    joblib.dump(reg, path.join(output_dir, experiment_name + '_model.sav'))
                    logger.info("save 1: {}".format(path.join(output_dir, experiment_name + '_model.sav')))


                logger.info("process data %s", time.time() - instant)

                client.complete_result(id)
                # Submit directory after completing a batch
                data = {
                    "id": new_id,
                    "size": 4,
                    "data_flag": {
                        "is_keep_data": False,
                        "is_transparent_data": False
                    },
                    "priority": 0,
                }
                client.submit_output(data)
                logger.info("submit new data %s", time.time() - instant)
            else:
                time.sleep(2)
                continue

        except IPCError as e:
            if e.code == 1:
                time.sleep(2)
                continue 
            elif e.code == 3: 
                client.finish()
                logger.info("incoming data finish")
                return 
            elif e.code == 4:
                logger.info("receive AlreadyFinish")
                time.sleep(60*60*24*365) # alway wait here to provent container restart
                return   
            elif e.code == 5:        
                logger.info("no avaiable data")
                time.sleep(2)
                continue
            else:
                logger.error("got unknown error %s", e)
                time.sleep(5)
if __name__ == "__main__":
    main()