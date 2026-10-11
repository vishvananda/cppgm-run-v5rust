void f(const int);
void f(volatile int);
void callback(int);
int data[3];
void g(void x(int), int* y);
void g(void (*)(int), int y[3]);
typedef int I;
int produced();
int variadic(...);
int nested(int (*)(int));
int f1(int ());
int f1(int (*)());
int f4(int (I (*)(I)));
int f4(int (*)(int (*)(int)));
int f5(int (...));
int f5(int (*)(...));
void check() {
  f(1);
  g(callback, data);
  f1(produced);
  f4(nested);
  f5(variadic);
}
