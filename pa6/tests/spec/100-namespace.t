// N3485 focus: 7.3.1 [namespace.def] namespace definitions
namespace N {
  typedef int Y[3];
  Y x;
  int f(Y y) { Y z; }
}
