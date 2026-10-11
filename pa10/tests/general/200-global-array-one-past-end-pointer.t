double data[100];
double *dpa = data;
double *dpb = data + 100;
int main() { return (dpb - dpa) == 100 ? 0 : 1; }
