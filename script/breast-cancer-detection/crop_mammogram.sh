#!/bin/bash

INPUT=$INPUT
OUTPUT=$OUTPUT

NUM_PROCESSES=10
DEVICE_TYPE=$1
NUM_EPOCHS=10
HEATMAP_BATCH_SIZE=100
GPU_NUMBER=0

ID=$(ls ${INPUT}/sample_data/ | head -n 1)
DATA_FOLDER="${INPUT}/sample_data/${ID}/"
INITIAL_EXAM_LIST_PATH="${INPUT}/sample_data/${ID}/gen_exam_list_before_cropping.pkl"
PATCH_MODEL_PATH="${INPUT}/models/sample_patch_model.p"
IMAGE_MODEL_PATH="${INPUT}/models/sample_image_model.p"
IMAGEHEATMAPS_MODEL_PATH="${INPUT}/models/sample_imageheatmaps_model.p"

CROPPED_IMAGE_PATH="${OUTPUT}/${ID}/cropped_images"
CROPPED_EXAM_LIST_PATH="${OUTPUT}/${ID}/cropped_images/cropped_exam_list.pkl"
EXAM_LIST_PATH="${OUTPUT}/${ID}/data.pkl"
HEATMAPS_PATH="${OUTPUT}/${ID}/heatmaps"
IMAGE_PREDICTIONS_PATH="${OUTPUT}/${ID}/image_predictions.csv"
IMAGEHEATMAPS_PREDICTIONS_PATH="${OUTPUT}/${ID}/imageheatmaps_predictions.csv"
export PYTHONPATH=$(pwd):$PYTHONPATH

echo 'Stage 1: Crop Mammograms'
python3 src/cropping/crop_mammogram.py \
    --input-data-folder $DATA_FOLDER \
    --output-data-folder $CROPPED_IMAGE_PATH \
    --exam-list-path $INITIAL_EXAM_LIST_PATH  \
    --cropped-exam-list-path $CROPPED_EXAM_LIST_PATH  \
    --num-processes $NUM_PROCESSES

