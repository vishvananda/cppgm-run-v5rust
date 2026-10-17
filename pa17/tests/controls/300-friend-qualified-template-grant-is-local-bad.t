// N3485 14.5.4p1: a qualified friend template grants its selected overload.
// expect reject
namespace N {
template<class T> int read(T);
template<class T> int read(T*);
}

class Owner {
  int value;
  template<class T> friend int N::read(T*);
};

namespace N {
template<class T> int read(T) { return Owner().value; }
}

int main() { return N::read(0); }
