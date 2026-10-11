namespace imported {
long f(int);
}
using namespace imported;
char f(int);
decltype(::f(0)) value;
