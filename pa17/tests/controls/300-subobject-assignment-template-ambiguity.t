// expect reject
struct Cell {
  template<class T, int N = 0> void operator=(T const&) volatile {}
  template<class T, long N = 0> void operator=(T const&) volatile {}
};
struct Store { volatile Cell cell; };
int main() { Store target{}, source{}; target = source; }

