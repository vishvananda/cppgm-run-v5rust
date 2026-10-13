int h(int);void g(int(&)(int));void g(int(*&&)(int));void f(){g(h);}
