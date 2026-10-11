namespace N {
template<typename T>
struct Box { T value; int get() { return value; } };
}

int main() { N::Box<int> box; box.value = 4; return box.get() - 4; }
