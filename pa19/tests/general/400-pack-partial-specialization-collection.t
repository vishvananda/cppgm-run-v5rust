// HHC-051: intended outcome: should compile successfully and collect the pack partial specialization.
template<class... T>
struct Box;

template<class T, class U, class... Rest>
struct Box<T, U, Rest...> {
};

int main() {
  return 0;
}
