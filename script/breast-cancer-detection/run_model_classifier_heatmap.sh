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

CROPPED_IMAGE_PATH="${INPUT}/${ID}/cropped_images"
CROPPED_EXAM_LIST_PATH="${INPUT}/${ID}/cropped_images/cropped_exam_list.pkl"
EXAM_LIST_PATH="${INPUT}/${ID}/data.pkl"
HEATMAPS_PATH="${INPUT}/${ID}/heatmaps"
IMAGE_PREDICTIONS_PATH="${INPUT}/${ID}/image_predictions.csv"
IMAGEHEATMAPS_PREDICTIONS_PATH="${INPUT}/${ID}/imageheatmaps_predictions.csv"
export PYTHONPATH=$(pwd):$PYTHONPATH

echo 'Stage 4b: Run Classifier (Image+Heatmaps)'
python3 src/modeling/run_model.py \
    --model-path $IMAGEHEATMAPS_MODEL_PATH \
    --data-path $EXAM_LIST_PATH \
    --image-path $CROPPED_IMAGE_PATH \
    --output-path $IMAGEHEATMAPS_PREDICTIONS_PATH \
    --use-heatmaps \
    --heatmaps-path $HEATMAPS_PATH \
    --use-augmentation \
    --num-epochs $NUM_EPOCHS \
    --device-type $DEVICE_TYPE \
    --gpu-number $GPU_NUMBER
