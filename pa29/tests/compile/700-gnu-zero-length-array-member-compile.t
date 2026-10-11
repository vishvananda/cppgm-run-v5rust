struct file_handle {
  unsigned int handle_bytes;
  int handle_type;
  unsigned char f_handle[0];
};

int main() {
  file_handle *h = 0;
  return h ? h->handle_type : 0;
}
