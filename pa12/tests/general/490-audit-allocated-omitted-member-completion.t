struct Member { Member(); ~Member(); };
struct Pair { Member first; Member second; };
struct Local { Local(); ~Local(); };
void f() { Local before; delete new Pair{}; }
