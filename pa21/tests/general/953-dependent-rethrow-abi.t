template<class T>auto raise(T value)->decltype((value,throw)){throw;}int main(){try{throw 3;}catch(...){try{raise(3);}catch(int x){return x-3;}}return 1;}
