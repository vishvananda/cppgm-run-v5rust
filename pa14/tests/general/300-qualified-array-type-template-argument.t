namespace std {
typedef unsigned int uint32_t;
}

template<class T>
struct box {
  static int size() {
    return sizeof(T);
  }
};

int main() {
  return box<std::uint32_t[4]>::size() == 16 ? 0 : 1;
}
