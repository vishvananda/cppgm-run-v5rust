template<typename T>
struct Box { T value; int get() { return value; } };

int main() { Box<int> box; box.value = 7; return box.get() - 7; }
