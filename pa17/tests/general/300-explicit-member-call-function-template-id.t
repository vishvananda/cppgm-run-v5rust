template<class T>
T& manip(T& x) {
  return x;
}

struct Sink {
  Sink& operator<<(Sink& (*pf)(Sink&)) {
    return pf(*this);
  }

  template<class U>
  Sink& operator<<(const U&) {
    return *this;
  }
};

int main() {
  Sink s;
  s.operator<<(manip);
  return 0;
}
