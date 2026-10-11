template <class T>
T&& my_move(T& value) {
  return static_cast<T&&>(value);
}

int copies = 0;
int moves = 0;

struct Box {
  int value;

  Box(int v) : value(v) {}

  Box(const Box& other) : value(other.value) {
    copies = copies + 1;
  }

  Box(Box&& other) : value(other.value) {
    moves = moves + 1;
    other.value = 0;
  }

  ~Box() {}
};

Box get(Box& source) {
  Box token = my_move(source);
  return token;
}

int main() {
  Box source(7);
  Box result = get(source);
  return result.value == 7 && source.value == 0 && moves == 1 && copies == 0 ? 0 : 1;
}
