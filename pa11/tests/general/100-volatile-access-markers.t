struct Device
{
  volatile int status;
  int data;
};

int poll(volatile int * port, Device & device)
{
  volatile int gate = 0;
  gate = 1;
  gate += 2;
  ++gate;
  int sampled = *port;
  sampled += device.status;
  device.status = sampled;
  device.data = sampled;
  volatile int * indirect = &gate;
  return *indirect + gate;
}

int main()
{
  Device device = {0, 0};
  Device devices[2] = {{0, 0}, {1, 2}};
  volatile int values[2][2] = {{3}, {4}};
  if (devices[0].status != 0 || devices[1].status != 1 ||
      values[0][0] != 3 || values[0][1] != 0 ||
      values[1][0] != 4 || values[1][1] != 0)
    return 1;
  int port = 5;
  return poll(const_cast<volatile int *>(&port), device) - 14;
}
