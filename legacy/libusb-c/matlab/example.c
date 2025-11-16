#include "serial_comm.h"

int main()
{
	usb_device_info devices[10];
	int count = 10;

	if (discover_devices(devices, &count))
	{
		if (open_device(&devices[0]))
		{
			uint8_t send_buf[] = "Hello";
			uint8_t recv_buf[64];
			int received;

			send_data(&devices[0], send_buf, sizeof(send_buf));
			receive_data(&devices[0], recv_buf, sizeof(recv_buf), &received);

			close_device(&devices[0]);
		}
	}
	return 0;
}