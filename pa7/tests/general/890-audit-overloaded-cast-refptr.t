int f(int);int f(double);void g(){static_cast<int(*const&)(int)>(f);}
