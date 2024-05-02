%global srcname aabl-copr
%global app_name an-anime-borb-launcher
%define install_dir %{_libdir}/%{app_name}
%define icon_dir %{_datadir}/icons/hicolor/512x512/apps
%define apps_dir %{_datadir}/applications
%define app_id moe.launcher.%{app_name}
%define build_output anime-borb-launcher

Name: an-anime-borb-launcher
Version: 1.0.1-rc1
Release: 1%{?dist}
License: GPLv3
Summary: Play the anime borb game in Linux.
Url: https://github.com/retrozinndev/%{srcname}
# Sources can be obtained by
# git clone https://github.com/retrozinndev/%{srcname}.git
# cd %{srcname}
# tito build --tgz
Source0: https://github.com/retrozinndev/%{srcname}/archive/refs/tags/v%{version}.tar.gz
BuildArch: x86_64

#-- APPLICATION DEPENDENCIES ---------------------------------------------------#
Requires: git
Requires: libcurl
Requires: libadwaita
Requires: gtk4
Requires: glibc

#-- OPTIONAL DEPENDENCIES ------------------------------------------------------#
Suggests: mangohud
Suggests: gamescope
Suggests: gamemode

#-- BUILD DEPENDENCIES ---------------------------------------------------------#
BuildRequires: rust
BuildRequires: cargo
BuildRequires: git
BuildRequires: libcurl
BuildRequires: libadwaita-devel
BuildRequires: gtk4-devel
BuildRequires: glibc
BuildRequires: glib2
BuildRequires: glib2-devel
BuildRequires: p7zip
BuildRequires: gcc
BuildRequires: gcc-c++
BuildRequires: cairo-devel
BuildRequires: gdk-pixbuf2-devel
BuildRequires: pango-devel
BuildRequires: rust-gdk4-devel

%description
Play the anime borb game in Linux.

#-- PREP, BUILD & INSTALL -----------------------------------------------------#
%prep
%autosetup

%build
cargo build --release

%install
# copy binary
mkdir -p %{buildroot}%{install_dir}
cp -f target/release/%{build_output} %{buildroot}%{install_dir}
# rename binary
mv %{buildroot}%{installdir}/%{build_output} %{buildroot}%{installdir}/%{name}
# copy icon
mkdir -p %{buildroot}%{icon_dir}
cp -f assets/images/%{app_id}.png %{buildroot}%{icon_dir}
# copy desktop file
mkdir -p %{buildroot}%{apps_dir}
cp -f assets/%{name}.desktop %{buildroot}%{apps_dir}

%post
# create link of binary
ln -sf %{install_dir}/%{name} %{_bindir}/%{name}
# apply exec permision to binary
chmod +x %{install_dir}/%{name}

#-- FILES ---------------------------------------------------------------------#
%files
%doc README.md
%license LICENSE
%{install_dir}/*
%{icon_dir}/%{app_id}.png
%{apps_dir}/%{name}.desktop

#-- CHANGELOG -----------------------------------------------------------------#
%changelog
