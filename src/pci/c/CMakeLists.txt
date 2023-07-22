cmake_minimum_required(VERSION 3.0)
project(kernel C)
add_library(ide STATIC ide.c)
install(TARGETS ide DESTINATION .)