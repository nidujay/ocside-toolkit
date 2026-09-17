// SPDX-License-Identifier: LGPL-3.0-or-later
#ifndef OCSIDE_WINDOWS_H
#define OCSIDE_WINDOWS_H

#define CALLBACK
#define WINAPI

#ifdef UNICODE

#define PWSTR LPCWSTR
#define WNDCLASS WNDCLASSW
#define WNDCLASSEX WNDCLASSEXW

#define RegisterClass RegisterClassW
#define DefWindowProc DefWindowProcW
#define CreateWindowEx CreateWindowExW
#define GetMessage GetMessageW
#define DispatchMessage DispatchMessageW

#else
#endif

#include <win32_impl.h>

#endif
