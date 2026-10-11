template<class Sig>
struct Box;

template<class R, class... Args>
struct Box<R(Args...)> {
  Box(Box const& b);
  int value;
};

template<class R, class... Args>
Box<R(Args...)>::Box(Box const& b) : value(b.value) {}

int main() {
  return 0;
}
