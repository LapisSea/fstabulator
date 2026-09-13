# Binary-only spec: scripts/install_to_rpm.sh stages the payload and passes
# cargo_version / fstab_payload via --define.
Name:		fstabulator
Version:	%{cargo_version}
Release:	1%{?dist}
Summary:	GTK4 GUI for editing /etc/fstab
License:	GPL-3.0-or-later
URL:		https://github.com/LapisSea/fstabulator
# Building the project on the host additionally needs cargo, gtk4-devel,
# libadwaita-devel, glib2-devel and gettext.

Requires:	gtk4 >= 4.12
Requires:	libadwaita >= 1.9
Requires:	glib2
Requires:	polkit
Requires:	util-linux
Recommends:	shadow-utils
Suggests:	btrfs-progs

%description
FSTabulator is a GTK4/libadwaita front end for /etc/fstab. It lists the
block devices and filesystems on the system, edits mount entries, keeps
timestamped backups, and applies changes (mount, remount, unmount, swap)
through a polkit-authenticated root helper.

%install
mkdir -p "%{buildroot}"
cp -a "%{fstab_payload}/." "%{buildroot}/"

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%{_bindir}/fstabulator
%{_datadir}/applications/org.lapissea.FSTabulator.desktop
%{_datadir}/icons/hicolor/scalable/apps/fstabulator.svg
%{_datadir}/icons/Adwaita-dark/scalable/apps/fstabulator.svg
%{_datadir}/icons/Adwaita-dark/index.theme
%{_datadir}/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy
%{_datadir}/locale/*/LC_MESSAGES/fstabulator.mo

%changelog
* Sat Aug 29 2026 LapisSea - 0.1.0-1
- Initial package
