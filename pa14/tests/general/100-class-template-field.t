template<typename T>
struct Box { T value; };

int main() { Box<int> box; box.value = 7; return box.value - 7; }
