struct Node { int value; int low; int high; };

bool outside(Node node) { return node.value < node.low || node.value > node.high; }
bool inside(Node node) { return node.value > node.low && node.value < node.high; }
