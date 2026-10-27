// Static null filling replays the first element's appended data items after
// merging adjacent zero ranges. The frozen writer's exact item bytes matter.
struct Target { int value; };
struct Element { long value; int Target::*member; };
struct Image { int prefix; Element elements[3]; };
Image image = {7};
int main() { return 0; }
