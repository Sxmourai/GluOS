cmake_minimum_required(VERSION 3.0)
project(ide C)
add_library(ide STATIC ide.c)
install(TARGETS ide DESTINATION .)