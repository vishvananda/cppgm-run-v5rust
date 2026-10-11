typedef int I;
int f1(int ());
int f3(int (*)(I));
int f4(int (I (*)(I)));
int f5(int (...));
int g1(int (x));
int g1(int);
int k(int (*const volatile)(void));
