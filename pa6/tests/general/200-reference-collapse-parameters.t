typedef int& LRI;
typedef int&& RRI;

void f(LRI&, const LRI&, const LRI&&, RRI&, RRI&&);

using LRR = LRI&&;
using RRL = RRI&;
using RRR = RRI&&;
extern LRR a;
extern RRL b;
extern RRR c;
