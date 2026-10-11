struct Box;

namespace detail {
void set(Box & box);
}

struct Box {
  Box() : value(0) {}

  int get() const {
    return value;
  }

private:
  int value;

  friend void detail::set(Box & box);
};

void detail::set(Box & box) {
  box.value = 7;
}

int main() {
  Box box;
  detail::set(box);
  return box.get() == 7 ? 0 : 1;
}
