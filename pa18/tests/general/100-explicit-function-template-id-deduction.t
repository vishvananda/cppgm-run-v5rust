namespace chrono {
  struct seconds {};
  struct nanoseconds {};

  template<class To, class From>
  To duration_cast(const From&);
}

using namespace chrono;

seconds f(const nanoseconds& ns) {
  return duration_cast<seconds>(ns);
}

int main() {
  return 0;
}
