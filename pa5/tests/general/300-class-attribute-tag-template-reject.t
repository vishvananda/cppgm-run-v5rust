template<class T> struct alignas(sizeof(struct Word*)) C{}; void f(){Word<int> *p;}
