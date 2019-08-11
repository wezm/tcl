set name ruby
set version 2.6.3
set ruby_abiver 2.6.0
set subdir [ replace $version \..* "" ]

pkgname $name
version $version
revision 2
build-style gnu-configure
configure_args --enable-shared --disable-rpath DOXYGEN /usr/bin/doxygen DOT /usr/bin/dot PKG_CONFIG /usr/bin/pkg-config
make_build_args all capi
hostmakedepends pkg-config bison groff
makedepends {
  zlib-devel
  readline-devel
  libffi-devel
  libressl-devel
  gdbm-devel
  libyaml-devel
  pango-devel
}
checkdepends tzdata
short_desc "Ruby programming language"
homepage http://www.ruby-lang.org/en/
maintainer "Wesley Moore <wes@wezm.net>"
license Ruby BSD-2-Clause
distfile https://cache.ruby-lang.org/pub/ruby/$subdir/$pkgname-$version.tar.bz2 {
  checksum dd638bf42059182c1d04af0d5577131d4ce70b79105231c4cc0a60de77b14f2e
}

