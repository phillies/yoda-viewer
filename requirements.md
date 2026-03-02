# Product vision

A tool similar to labelme but it's used via browser and optimized for Ultralytics YOLO style labels
I want to review and edit segmentation labels via browser ui.

## Features
### Version 1 ✅
- Can read images and labels
- Can display images with it's labels
- Labels can be shown as bounding box and/or segmentation mask
- User can provide a color map for class to color
- User can provide a map cladd ID to class name
- App is developed with NiceGUI
- Images and labels are available in local folders, separate folders for images and labels but substructure of both folders is identical
- Image file and label file have the same path and filename relative to the image and label base folder, respectively. Only difference is the file extension, labels have .txt, images have .jpg, .jpeg, .png
- A file tree view on the left side shows the root image folder and all subfolders and files
- A click on the file opens the image with label in the main window
- On the right are controls for showing bounding box, segmentation mask, class ID, class Name
- On the right is a list of all objects with their class

### Version 2 ✅
- Can change the class of a label
- In the list on the right you can select which classes to show / not show
- You can hide objects via the list on the right


### Version 3 ✅
- Can draw new objects by clicking the "add object" button
- When the button is clicked, you can add vertices by clicking into the images. The vertices connect to a segmentation mask. When ESC is clicked, the vertices are discarded, when the first vertex is clicked or enter is presses, the segmentation mask is stored.
- To switch back from add object to edit mode, there is a button with a hand (like the move button in other image editing apps)
- Can delete objects by selecting them and then click a delete button


## Guidelines
- Example data is in the example_data folder, use this for testing
- color_map.yaml is a sample color map for testing
