int source() { return 2; }

template<class T> int value = source();

int *objects[2] = {&value<int>, &value<char>};

int main() {
  return *objects[0] != 2 || *objects[1] != 2;
}
