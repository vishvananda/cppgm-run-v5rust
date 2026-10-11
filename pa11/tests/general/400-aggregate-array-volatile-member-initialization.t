struct Device
{
  volatile int status;
  int data;
};
int main()
{
  Device devices[2] = {{3, 1}, {4, 2}};
  int first = devices[0].status;
  devices[1].status = 8;
  return first == 3 && devices[1].status == 8 &&
         devices[0].data == 1 && devices[1].data == 2 ? 0 : 1;
}
