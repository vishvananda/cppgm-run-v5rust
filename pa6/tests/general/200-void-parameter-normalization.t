extern int pthread_once(int*, void (*)(void));

void init() {}

void call(int* x) {
  pthread_once(x, init);
}
