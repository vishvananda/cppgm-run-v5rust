// Parser categories: redeclarations retain templates; member lookup uses member marks.
template<class T> int get();
int get();
struct C { template<class T> int count(); };
int f() {
  get<int>();
  x.count<int>();
  x.count < a || x.count > (b);
  x.get < a || x.get > (b);
  return 0;
}

struct Owner { friend struct Unknown; };
void g() { Unknown * p; }
