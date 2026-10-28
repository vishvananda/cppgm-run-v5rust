__attribute__((noinline)) void pressure(volatile unsigned long* p,
                                       volatile unsigned long* q, double) {
  unsigned long a0=*p, a1=*p, a2=*p, a3=*p;
  unsigned long a4=*p, a5=*p, a6=*p;
  (void)*reinterpret_cast<volatile unsigned char*>(p);
  *q=a0; *q=a1; *q=a2; *q=a3; *q=a4; *q=a5; *q=a6;
}
int main() {
  volatile unsigned long source=17, result=0;
  pressure(&source, &result, 1.25);
  return result==17?0:1;
}
