int f(int);int f(double);void g(){int(*p)(int)=&(*f);}
